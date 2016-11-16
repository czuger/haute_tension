class AdventuresController < ApplicationController
  before_action :set_adventure, only: [:show, :edit, :update, :destroy]

  # GET /aventures
  # GET /adventures.json
  def index
    @adventures = Adventure.all
  end

  # GET /adventures/1
  # GET /adventures/1.json
  def show
  end

  def play
    adventure = Adventure.find(params[:adventure_id])
    @page = adventure.page
    # to_i.to_s avoid injections
    @page.text.each{ |text| text.gsub!( 'CHANGE_ADVENTURE_ID', params[:adventure_id].to_i.to_s ) }
  end

  def fight
  end

  def read_choice
    adventure = Adventure.find(params[:adventure_id])
    adventure.page_id = params[:page_id]
    adventure.save!
    redirect_to adventure_play_url(adventure )
  end

  # GET /adventures/new
  def new
    @adventure = Adventure.new
  end

  # GET /adventures/1/edit
  def edit
  end

  # POST /adventures
  # POST /adventures.json
  def create
    @adventure = Adventure.new(adventure_params)

    book = Book.find( adventure_params[ 'book_id' ] )
    @adventure.page = book.first_page

    roll_adventure

    respond_to do |format|
      if @adventure.save
        format.html { redirect_to @adventure, notice: 'Aventure was successfully created.' }
        format.json { render :show, status: :created, location: @adventure }
      else
        format.html { render :new }
        format.json { render json: @adventure.errors, status: :unprocessable_entity }
      end
    end
  end

  def reroll
    @adventure = Adventure.find(params[:adventure_id])
    roll_adventure
    adventure.save!
    redirect_to adventure
  end

  def roll_dices
    @adventure = Adventure.find(params[:adventure_id])
    @dices = 1.upto( 2 ).inject{ |s| s + rand( 1..6 ) }
  end

  # PATCH/PUT /adventures/1
  # PATCH/PUT /adventures/1.json
  def update
    respond_to do |format|
      if @adventure.update(adventure_params)
        format.html { redirect_to @adventure, notice: 'Aventure was successfully updated.' }
        format.json { render :show, status: :ok, location: adventure }
      else
        format.html { render :edit }
        format.json { render json: @adventure.errors, status: :unprocessable_entity }
      end
    end
  end

  # DELETE /adventures/1
  # DELETE /adventures/1.json
  def destroy
    @adventure.destroy
    respond_to do |format|
      format.html { redirect_to adventures_url, notice: 'Aventure was successfully destroyed.' }
      format.json { head :no_content }
    end
  end

  private
    # Use callbacks to share common setup or constraints between actions.
    def set_adventure
      @adventure = Adventure.find(params[:id])
    end

    # Never trust parameters from the scary internet, only allow the white list through.
    def adventure_params
      params.require(:adventure).permit(:book_id, :adventure_id, :page_id, :hp, :gold, :gourdes, :gourdes_remplies, :rations, :charisme )
    end

    def roll_adventure
      @adventure.hp = 18 + 1.upto(2 ).inject{ |s| s + rand(1..6 ) }
      @adventure.force = 6 + 1.upto(2 ).inject{ |s| s + rand(1..6 ) }
      @adventure.force = 19 if @adventure.force == 17
      @adventure.force = 20 if @adventure.force == 18
      @adventure.gold = 1.upto(4 ).inject{ |s| s + rand(1..6 ) }
    end
end
