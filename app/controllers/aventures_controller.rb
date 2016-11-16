class AventuresController < ApplicationController
  before_action :set_aventure, only: [:show, :edit, :update, :destroy]

  # GET /aventures
  # GET /aventures.json
  def index
    @aventures = Aventure.all
  end

  # GET /aventures/1
  # GET /aventures/1.json
  def show
  end

  def play
    @aventure = Aventure.find(params[:aventure_id])
    @page = @aventure.page
    # to_i.to_s avoid injections
    @page.text.each{ |text| text.gsub!( 'CHANGE_ADVENTURE_ID', params[:aventure_id].to_i.to_s ) }
  end

  def fight
  end

  def read_choice
    @aventure = Aventure.find(params[:aventure_id])
    @aventure.page_id = params[:page_id]
    @aventure.save!
    redirect_to aventure_play_url( @aventure )
  end

  # GET /aventures/new
  def new
    @aventure = Aventure.new
  end

  # GET /aventures/1/edit
  def edit
  end

  # POST /aventures
  # POST /aventures.json
  def create
    @aventure = Aventure.new(aventure_params)

    p aventure_params

    book = Book.find( aventure_params[ 'book_id' ] )
    @aventure.page = book.first_page

    roll_aventure

    respond_to do |format|
      if @aventure.save
        format.html { redirect_to @aventure, notice: 'Aventure was successfully created.' }
        format.json { render :show, status: :created, location: @aventure }
      else
        format.html { render :new }
        format.json { render json: @aventure.errors, status: :unprocessable_entity }
      end
    end
  end

  def reroll
    @aventure = Aventure.find(params[:aventure_id])
    roll_aventure
    @aventure.save!
    redirect_to @aventure
  end

  def roll_dices
    @aventure = Aventure.find(params[:aventure_id])
    @dices = 1.upto( 2 ).inject{ |s| s + rand( 1..6 ) }
  end

  # PATCH/PUT /aventures/1
  # PATCH/PUT /aventures/1.json
  def update
    respond_to do |format|
      if @aventure.update(aventure_params)
        format.html { redirect_to @aventure, notice: 'Aventure was successfully updated.' }
        format.json { render :show, status: :ok, location: @aventure }
      else
        format.html { render :edit }
        format.json { render json: @aventure.errors, status: :unprocessable_entity }
      end
    end
  end

  # DELETE /aventures/1
  # DELETE /aventures/1.json
  def destroy
    @aventure.destroy
    respond_to do |format|
      format.html { redirect_to aventures_url, notice: 'Aventure was successfully destroyed.' }
      format.json { head :no_content }
    end
  end

  private
    # Use callbacks to share common setup or constraints between actions.
    def set_aventure
      @aventure = Aventure.find(params[:id])
    end

    # Never trust parameters from the scary internet, only allow the white list through.
    def aventure_params
      params.require(:aventure).permit(:book_id, :aventure_id, :page_id, :hp, :gold, :gourdes, :gourdes_remplies, :rations, :charisme )
    end

    def roll_aventure
      @aventure.hp = 18 + 1.upto( 2 ).inject{ |s| s + rand( 1..6 ) }
      @aventure.force = 6 + 1.upto( 2 ).inject{ |s| s + rand( 1..6 ) }
      @aventure.force = 19 if @aventure.force == 17
      @aventure.force = 20 if @aventure.force == 18
      @aventure.gold = 1.upto( 4 ).inject{ |s| s + rand( 1..6 ) }
    end
end
